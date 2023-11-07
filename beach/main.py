import csv
import os
import sys

import psycopg2


def export(data_dir: str, batch_size: int = 50):
    connection = psycopg2.connect(os.environ["DATABASE_URL"])
    cursor = connection.cursor()
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
    cursor.close()
    connection.close()


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python main.py ./data/")
        exit(1)
    export(sys.argv[1])
