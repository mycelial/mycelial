.PHONY: bootstrap
bootstrap:
	cd ../ && git clone github.com/mycelial/pipexperiments || true
	cd ../pipexperiments/ && git pull
