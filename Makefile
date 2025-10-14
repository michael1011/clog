db-start:
	docker run --name ctrack-db \
		--rm \
		-e POSTGRES_DB=ctrack \
		-e POSTGRES_USER=ctrack \
		-e POSTGRES_PASSWORD=ctrack \
		-d -p 5433:5432 postgres:18-alpine

db-stop:
	docker stop ctrack-db
