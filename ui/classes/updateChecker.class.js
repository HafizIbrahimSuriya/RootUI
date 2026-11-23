// RootUI has no upstream release feed (the original checked eDEX-UI's GitHub releases).
class UpdateChecker {
    constructor() {
        this._failed = false;
        this._willfail = false;
    }
}

module.exports = {
    UpdateChecker
};
