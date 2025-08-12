// Test file
import { getUserById } from './user.js';

/**
 * Test get user functionality
 */
export function testGetUser() {
    const user = getUserById('test-id');
    console.log('User:', user);
    return user;
}

/**
 * Test multiple users
 */
export function testMultipleUsers() {
    const user1 = getUserById('1');
    const user2 = getUserById('2');
    return [user1, user2];
}