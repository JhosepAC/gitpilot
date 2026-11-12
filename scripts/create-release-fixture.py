"""Create synthetic release QA data in an explicitly supplied, unused database directory."""

import argparse
import hashlib
from pathlib import Path
import sqlite3
import subprocess


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--data-dir", type=Path, required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--through", type=int, default=999999999999)
    args = parser.parse_args()
    args.data_dir.mkdir(parents=True, exist_ok=True)
    database = args.data_dir / "gitpilot.sqlite3"
    if database.exists():
        raise SystemExit("Refusing to replace an existing QA database")
    migrations = Path(__file__).resolve().parents[1] / "apps/desktop/src-tauri/migrations"
    with sqlite3.connect(database) as connection:
        connection.execute("CREATE TABLE _sqlx_migrations(version BIGINT PRIMARY KEY, description TEXT NOT NULL, installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, success BOOLEAN NOT NULL, checksum BLOB NOT NULL, execution_time BIGINT NOT NULL)")
        for migration in sorted(migrations.glob("*.sql")):
            version, description = migration.stem.split("_", 1)
            if int(version) > args.through:
                continue
            sql = migration.read_text(encoding="utf-8")
            connection.executescript(sql)
            connection.execute("INSERT INTO _sqlx_migrations(version,description,success,checksum,execution_time) VALUES(?,?,1,?,0)",
                               (int(version), description.replace("_", " "), hashlib.sha384(sql.encode()).digest()))
        for number, label in enumerate(["Personal QA", "Empresa QA", "Sin asignaciones QA"], 1):
            connection.execute("INSERT INTO identities(id,label,git_name,email,is_default) VALUES(?,?,?,?,?)",
                               (number, label, "Autor QA", f"qa{number}@example.test", number == 1))
        paths = [args.workspace / "personal/Proyectos", args.workspace / "empresa/Proyectos", args.workspace / "personal/Proyectos/Equipo"]
        for number, path in enumerate(paths, 1):
            path.mkdir(parents=True, exist_ok=True)
            connection.execute("INSERT INTO directories(id,path) VALUES(?,?)", (number, str(path)))
            connection.execute("INSERT INTO identity_rules(identity_id,directory_id) VALUES(?,?)", (2 if number == 2 else 1, number))
        repo = paths[2] / "Repositorio de prueba"
        if repo.exists():
            raise SystemExit("Refusing to reuse an existing QA repository")
        subprocess.run(["git", "init", "--quiet", str(repo)], check=True)
        connection.execute("INSERT INTO repositories(id,path,kind,git_dir,common_dir) VALUES(1,?,'ordinary',?,?)",
                           (str(repo), str(repo / ".git"), str(repo / ".git")))
        connection.execute("INSERT INTO identity_rules(identity_id,repository_id) VALUES(1,1)")
        connection.executemany("INSERT INTO directory_repositories(directory_id,repository_id,present) VALUES(?,1,1)", [(1,), (3,)])
    print("Synthetic database created:", database)


if __name__ == "__main__":
    main()
