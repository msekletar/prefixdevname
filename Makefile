NAME:=prefixdevname
MAJOR:=0
MINOR:=2
PATCH:=0

VERSION:=$(MAJOR).$(MINOR).$(PATCH)
ARCHIVE:=$(NAME)-$(VERSION).tar.gz
VENDOR:=$(NAME)-$(VERSION)-vendor.tar.gz

FEDORA_VERSION:=rawhide

all: release

debug:
	@cargo build

release:
	@cargo build --release

.PHONY: check install uninstall dist srpm rpm clean vendor

check:
	@unshare -m -u -r test/test.sh

install:
	mkdir -p $(DESTDIR)/usr/lib/udev/rules.d
	mkdir -p $(DESTDIR)/usr/lib/dracut/modules.d/71prefixdevname
	mkdir -p $(DESTDIR)/usr/lib/dracut/modules.d/71prefixdevname-tools
	install -p -m 0755 target/release/$(NAME) $(DESTDIR)/usr/lib/udev/
	install -p -m 644 rules/71-prefixdevname.rules $(DESTDIR)/usr/lib/udev/rules.d/
	install -p -m 0755 dracut/71prefixdevname/module-setup.sh $(DESTDIR)/usr/lib/dracut/modules.d/71prefixdevname/
	install -p -m 0755 dracut/71prefixdevname-tools/module-setup.sh $(DESTDIR)/usr/lib/dracut/modules.d/71prefixdevname-tools/

uninstall:
	rm -f $(DESTDIR)/usr/lib/udev/$(NAME)
	rm -f $(DESTDIR)/usr/lib/udev/rules/71-prefixdevname.rules
	rm -rf $(DESTDIR)/usr/lib/dracut/modules.d/71prefixdevname

dist:
	@git archive HEAD --prefix $(NAME)-$(VERSION)/ | gzip > $(ARCHIVE)

vendor:
	@rm -rf vendor
	@cargo vendor
	@rm -f vendor/winapi*/lib/*
	@tar -czf $(VENDOR) vendor

srpm: dist
	@cp $(ARCHIVE) $(VENDOR) ~/rpmbuild/SOURCES
	@cp redhat/$(NAME).spec ~/rpmbuild/SPECS
	@rpmbuild -bs ~/rpmbuild/SPECS/$(NAME).spec

rpm: srpm
	$(eval SRPM:=$(shell ls ~/rpmbuild/SRPMS/$(NAME)*.src.rpm))
	@rpmbuild --rebuild $(SRPM)

mock-rpm: srpm
	$(eval SRPM:=$(shell ls ~/rpmbuild/SRPMS/$(NAME)*.src.rpm))
	@mock -r --rebuild $(SRPM)

clean:
	@cargo clean
	@rm -f $(ARCHIVE)
