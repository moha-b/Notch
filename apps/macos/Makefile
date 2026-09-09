PROJECT := Notch.xcodeproj
SCHEME := Notch
DEST := platform=macOS,arch=arm64
SIGN := CODE_SIGN_IDENTITY="-" DEVELOPMENT_TEAM=""

.PHONY: gen build test test-ci candidate
gen:
	xcodegen generate
build: gen
	xcodebuild -project $(PROJECT) -scheme $(SCHEME) -destination '$(DEST)' -configuration Debug $(SIGN) build
test: gen
	xcodebuild -project $(PROJECT) -scheme $(SCHEME) -destination '$(DEST)' -configuration Debug $(SIGN) test
test-ci: gen
	xcodebuild -project $(PROJECT) -scheme $(SCHEME) -destination '$(DEST)' -configuration Debug -derivedDataPath build/DerivedData -resultBundlePath build/tests.xcresult CODE_SIGNING_ALLOWED=NO test
candidate: gen
	xcodebuild -project $(PROJECT) -scheme $(SCHEME) -destination '$(DEST)' -configuration Release -derivedDataPath build/DerivedData $(SIGN) build
	mkdir -p build/dmg-stage
	cp -R build/DerivedData/Build/Products/Release/Notch.app build/dmg-stage/
	ln -s /Applications build/dmg-stage/Applications
	hdiutil create -volname 'Notch Preview' -srcfolder build/dmg-stage -format UDZO build/Notch-Preview.dmg
