// テスト用ファイル - 変更後のバージョン（破壊的変更）
// getUserByIdのシグネチャ変更とdeleteUser削除
function getUserById(id, options) {
    return database.get(id, options);
}

// deleteUser関数が削除された！

function updateUser(id, data) {
    return database.update(id, data);
}

module.exports = { getUserById, updateUser };
