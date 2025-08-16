// テスト用ファイル - 変更前のバージョン
function getUserById(id) {
    return database.get(id);
}

function deleteUser(id) {
    return database.delete(id);
}

function updateUser(id, data) {
    return database.update(id, data);
}

module.exports = { getUserById, deleteUser, updateUser };