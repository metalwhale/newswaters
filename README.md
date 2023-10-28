# newswaters
In the ocean, there are no newspapers. Whales refer to it as "newswaters".

## Local development
### Startup
1. Create a `.env` file by copying from [`./local.env`](./local.env):
    ```bash
    cp local.env .env
    ```
    and then fill in the variables with appropriate values in the `.env` file.
2. Start the containers:
    ```bash
    docker-compose up -d
    ```

### Run the echolocator
Whales learn about their surrounding environment by echolocating.
1. Get inside the container:
    ```bash
    docker-compose exec echolocator bash
    ```
2. Run the server:
    ```bash
    cargo run
    ```
3. Instruct:
    ```bash
    curl -X POST http://localhost:3000/instruct \
        -H 'Content-Type: application/json' \
        -d '{"instruction":"Sing a song"}'
    ```

### Run the skimmer
Whales feed by skimming.
1. Get inside the container:
    ```bash
    docker-compose exec skimmer bash
    ```
2. Run migrations:
    ```bash
    DATABASE_URL=postgres://${DATABASE_USER}:${DATABASE_PASSWORD}@${DATABASE_HOST}:${DATABASE_PORT}/${DATABASE_DB} diesel migration run
    ```
3. Collect items:
    ```bash
    SKIMMER_IS_JOB=true cargo run -- collect_items
    ```
    Or item urls:
    ```bash
    SKIMMER_IS_JOB=true cargo run -- collect_item_urls
    ```
4. Consume top stories:
    ```bash
    SKIMMER_IS_JOB=true cargo run -- consume_top_stories
    ```

### Run the whistler
Whales communicate through whistling.
1. Get inside the container:
    ```bash
    docker-compose exec whistler bash
    ```

## References
- [Building LLM applications for production](https://huyenchip.com/2023/04/11/llm-engineering.html)
- [Patterns for Building LLM-based Systems & Products](https://eugeneyan.com/writing/llm-patterns/)
- [LLM Applications](https://github.com/ray-project/llm-applications)
- [Optimizing your LLM in production](https://huggingface.co/blog/optimize-llm)
- [Fine-tuning a masked language model](https://huggingface.co/learn/nlp-course/chapter7/3)
