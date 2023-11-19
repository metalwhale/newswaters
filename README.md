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

### Run the search engine
1. Get inside the container:
    ```bash
    docker-compose exec search-engine bash
    ```
2. Run the server:
    ```bash
    cargo run
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
    Or if you are running on `arm64` ([reference](https://github.com/huggingface/candle/issues/494#issuecomment-1682919922)):
    ```bash
    RUSTFLAGS='-C target-feature=+fp16' cargo run
    ```
3. Instruct:
    ```bash
    curl -X POST http://localhost:3000/instruct \
        -H 'Content-Type: application/json' \
        -d '{"instruction":"Sing a song"}'
    ```
    Or embed:
    ```bash
    curl -X POST http://localhost:3000/embed \
        -H 'Content-Type: application/json' \
        -d '{"sentence":"Sing a song"}'
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
    SKIMMER_IS_JOB=true cargo run -- collect_item_urls
    ```
4. Summarize texts and embed summaries:
    ```bash
    SKIMMER_IS_JOB=true cargo run -- summarize_texts
    SKIMMER_IS_JOB=true cargo run -- embed_summaries
    ```
    Analyze texts and embed keywords:
    ```bash
    SKIMMER_IS_JOB=true cargo run -- analyze_texts
    SKIMMER_IS_JOB=true cargo run -- embed_keywords
    ```
    Analyze summaries:
    ```bash
    SKIMMER_IS_JOB=true cargo run -- analyze_summaries
    ```

### Run the whistler
Whales communicate through whistling.
1. Get inside the container:
    ```bash
    docker-compose exec whistler bash
    ```
2. Run the server:
    ```bash
    cargo run
    ```
3. Search similar items:
    ```bash
    curl -X POST http://localhost:3000/search-similar-items \
        -H 'Content-Type: application/json' \
        -d '{"sentence":"machine learning", "limit": 20}'
    ```

## References
### Blogs
- [Building LLM applications for production](https://huyenchip.com/2023/04/11/llm-engineering.html)
- [Patterns for Building LLM-based Systems & Products](https://eugeneyan.com/writing/llm-patterns/)
- [Optimizing your LLM in production](https://huggingface.co/blog/optimize-llm)
- [Fine-tuning a masked language model](https://huggingface.co/learn/nlp-course/chapter7/3)

### Repositories
- [LLM Applications](https://github.com/ray-project/llm-applications)
- [Demystifying Advanced RAG Pipelines](https://github.com/pchunduri6/rag-demystified)
