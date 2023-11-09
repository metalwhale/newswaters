import csv
import os
import sys

import psycopg2


def export_texts(cursor: psycopg2.extensions.cursor, data_dir: str, batch_size: int = 50):
    cursor.execute("SELECT count(*) FROM item_urls WHERE text IS NOT NULL")
    total, *_ = cursor.fetchone()
    print(f"Total: {total}")
    # batch_index = total // batch_size  # Temporary
    batch_index = 0
    texts_dir = os.path.join(data_dir, "texts")
    os.makedirs(texts_dir, exist_ok=True)
    with open(os.path.join(data_dir, "items.csv"), "w") as items_file:
        items_writer = csv.DictWriter(
            items_file, fieldnames=["id", "url", "title"])
        items_writer.writeheader()
        while True:
            print(f"[INFO] batch_index={batch_index}")
            offset = batch_index * batch_size
            cursor.execute(
                "SELECT id, url, title, item_urls.text "
                "FROM items "
                "JOIN item_urls ON items.id = item_urls.item_id "
                "WHERE item_urls.text IS NOT NULL "
                "ORDER BY id DESC "
                f"OFFSET {offset} LIMIT {batch_size}"
            )
            results = cursor.fetchall()
            if len(results) == 0:
                break
            for row in results:
                item_id, url, title, text = row
                items_writer.writerow({"id": item_id, "url": url, "title": title})
                with open(os.path.join(texts_dir, f"{item_id}.txt"), "w") as text_file:
                    text_file.write(text)
            items_file.flush()
            batch_index += 1


def export_summaries(cursor: psycopg2.extensions.cursor, data_dir: str, batch_size: int = 50):
    cursor.execute("SELECT count(*) FROM item_urls WHERE summary IS NOT NULL")
    total, *_ = cursor.fetchone()
    print(f"Total: {total}")
    batch_index = 0
    summaries_dir = os.path.join(data_dir, "summaries")
    os.makedirs(summaries_dir, exist_ok=True)
    with open(os.path.join(data_dir, "items.csv"), "w") as items_file:
        items_writer = csv.DictWriter(
            items_file, fieldnames=["id", "title"])
        items_writer.writeheader()
        while True:
            print(f"[INFO] batch_index={batch_index}")
            offset = batch_index * batch_size
            cursor.execute(
                "SELECT id, title, summary "
                "FROM items "
                "JOIN item_urls ON items.id = item_urls.item_id "
                "WHERE summary IS NOT NULL "
                "ORDER BY id DESC "
                f"OFFSET {offset} LIMIT {batch_size}"
            )
            results = cursor.fetchall()
            if len(results) == 0:
                break
            for row in results:
                item_id, title, summary = row
                items_writer.writerow({"id": item_id, "title": title})
                with open(os.path.join(summaries_dir, f"{item_id}.txt"), "w") as summary_file:
                    summary_file.write(summary)
            items_file.flush()
            batch_index += 1


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python main.py <COMMAND> ./data/")
        exit(1)
    command, data_dir = sys.argv[1:]
    connection = psycopg2.connect(os.environ["DATABASE_URL"])
    cursor = connection.cursor()
    if command == "texts":
        export_texts(cursor, data_dir)
    elif command == "summaries":
        export_summaries(cursor, data_dir)
    cursor.close()
    connection.close()
