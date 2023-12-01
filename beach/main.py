import json
import os
from pathlib import Path

import psycopg2


def export_passages(cursor: psycopg2.extensions.cursor, data_dir: str, chunk_size: int = 50):
    cursor.execute("SELECT count(*) FROM analyses WHERE text_passage IS NOT NULL OR summary_passage IS NOT NULL")
    total, *_ = cursor.fetchone()
    print(f"Total: {total}")
    chunk_index = 0
    os.makedirs(data_dir, exist_ok=True)
    while True:
        print(f"[INFO] chunk_index={chunk_index}")
        offset = chunk_index * chunk_size
        cursor.execute(
            "SELECT id, items.text, item_urls.summary, text_passage, summary_passage "
            "FROM items "
            "LEFT JOIN item_urls ON items.id = item_urls.item_id "
            "LEFT JOIN analyses ON items.id = analyses.item_id "
            "WHERE text_passage IS NOT NULL OR summary_passage IS NOT NULL "
            "ORDER BY id DESC "
            f"OFFSET {offset} LIMIT {chunk_size}"
        )
        results = cursor.fetchall()
        if len(results) == 0:
            break
        for row in results:
            item_id, text, summary, text_passage, summary_passage = row
            with open(os.path.join(data_dir, f"{item_id}.json"), "w") as item_file:
                if text_passage is not None:
                    content = text
                    passage = text_passage
                elif summary_passage is not None:
                    content = summary
                    passage = summary_passage
                else:
                    continue
                item_data = {
                    "content": content,
                    "passage": json.loads(passage),
                }
                item_file.write(json.dumps(item_data, indent=4))
        chunk_index += 1


if __name__ == "__main__":
    connection = psycopg2.connect(os.environ["DATABASE_URL"])
    cursor = connection.cursor()
    export_passages(cursor, os.path.join(Path(__file__).parent, "data"))
    cursor.close()
    connection.close()
