clean:
	@echo ""
	@echo "------- Target 'samples/clean'" starting
	@find . -name \*.wasm -exec rm -rf {} + ; true
	@find . -name test.output -exec rm -rf {} + ; true
	@find . -name failed.output -exec rm -rf {} + ; true
	@find . -name manifest.json -exec rm -rf {} + ; true
	@find . -name \*.dump -type f -exec rm -rf {} + ; true
	@find . -name \*.txt -type f -exec rm -rf {} + ; true
	@find . -name \*.dot -type f -exec rm -rf {} + ; true
	@find . -name \*.dot.png -type f -exec rm -rf {} + ; true
	@echo "------- Target 'samples/clean'" done
