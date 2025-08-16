// deleteUser function removed and getUserById signature changed
// テスト用ファイル - 変更前のバージョン（オリジナル）
function getUserById(id, options) {
    return database.get(id);
}

    return database.delete(id);
}

function updateUser(id, data) {
    return database.update(id, data);
}

module.exports = { getUserById, deleteUser, updateUser };
