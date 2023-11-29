import csv
import json
import os
import sys

import psycopg2


def export_texts(cursor: psycopg2.extensions.cursor, data_dir: str, chunk_size: int = 50):
    cursor.execute("SELECT count(*) FROM item_urls WHERE text IS NOT NULL")
    total, *_ = cursor.fetchone()
    print(f"Total: {total}")
    # chunk_index = total // chunk_size  # Temporary
    chunk_index = 0
    texts_dir = os.path.join(data_dir, "texts")
    os.makedirs(texts_dir, exist_ok=True)
    with open(os.path.join(data_dir, "items.csv"), "w") as items_file:
        items_writer = csv.DictWriter(
            items_file, fieldnames=["id", "url", "title"])
        items_writer.writeheader()
        while True:
            print(f"[INFO] chunk_index={chunk_index}")
            offset = chunk_index * chunk_size
            cursor.execute(
                "SELECT id, url, title, item_urls.text "
                "FROM items "
                "JOIN item_urls ON items.id = item_urls.item_id "
                "WHERE item_urls.text IS NOT NULL "
                "ORDER BY id DESC "
                f"OFFSET {offset} LIMIT {chunk_size}"
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
            chunk_index += 1


def export_summaries(cursor: psycopg2.extensions.cursor, data_dir: str, chunk_size: int = 50):
    cursor.execute("SELECT count(*) FROM item_urls WHERE summary IS NOT NULL")
    total, *_ = cursor.fetchone()
    print(f"Total: {total}")
    chunk_index = 0
    summaries_dir = os.path.join(data_dir, "summaries")
    os.makedirs(summaries_dir, exist_ok=True)
    with open(os.path.join(data_dir, "items.csv"), "w") as items_file:
        items_writer = csv.DictWriter(
            items_file, fieldnames=["id", "title"])
        items_writer.writeheader()
        while True:
            print(f"[INFO] chunk_index={chunk_index}")
            offset = chunk_index * chunk_size
            cursor.execute(
                "SELECT id, title, summary "
                "FROM items "
                "JOIN item_urls ON items.id = item_urls.item_id "
                "WHERE summary IS NOT NULL "
                "ORDER BY id DESC "
                f"OFFSET {offset} LIMIT {chunk_size}"
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
            chunk_index += 1


def export_queries(cursor: psycopg2.extensions.cursor, data_dir: str, chunk_size: int = 50):
    cursor.execute("SELECT count(*) FROM analyses WHERE summary_passage IS NOT NULL")
    total, *_ = cursor.fetchone()
    print(f"Total: {total}")
    chunk_index = 0
    queries_dir = os.path.join(data_dir, "queries")
    os.makedirs(queries_dir, exist_ok=True)
    with open(os.path.join(data_dir, "items.csv"), "w") as items_file:
        items_writer = csv.DictWriter(
            items_file, fieldnames=["id", "title"])
        items_writer.writeheader()
        while True:
            print(f"[INFO] chunk_index={chunk_index}")
            offset = chunk_index * chunk_size
            cursor.execute(
                "SELECT id, title, summary, keyword, summary_passage "
                "FROM items "
                "LEFT JOIN item_urls ON items.id = item_urls.item_id "
                "LEFT JOIN analyses ON items.id = analyses.item_id "
                "WHERE summary_passage IS NOT NULL "
                "ORDER BY id DESC "
                f"OFFSET {offset} LIMIT {chunk_size}"
            )
            results = cursor.fetchall()
            if len(results) == 0:
                break
            for row in results:
                item_id, title, summary, keyword, passage = row
                items_writer.writerow({"id": item_id, "title": title})
                with open(os.path.join(queries_dir, f"summary_{item_id}.txt"), "w") as summary_file, \
                        open(os.path.join(queries_dir, f"keyword_{item_id}.txt"), "w") as keyword_file, \
                        open(os.path.join(queries_dir, f"passage_{item_id}.json"), "w") as passage_file:
                    summary_file.write(summary + "\n")
                    keyword_file.write(keyword + "\n")
                    passage_file.write(json.dumps(json.loads(passage), indent=4))
            items_file.flush()
            chunk_index += 1


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
    elif command == "queries":
        export_queries(cursor, data_dir)
    cursor.close()
    connection.close()
