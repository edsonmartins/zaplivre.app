import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { invoke } from '@tauri-apps/api/core'
import { formatDate } from '../utils/format'

interface Group {
  id: string
  name: string
  description: string | null
  member_count: number
  is_admin: boolean
  created_at: number
}

interface GroupListViewProps {
  localPeerId: string | null
}

export default function GroupListView({ localPeerId }: GroupListViewProps) {
  const [groups, setGroups] = useState<Group[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [showCreateDialog, setShowCreateDialog] = useState(false)
  const [groupName, setGroupName] = useState('')
  const [groupDescription, setGroupDescription] = useState('')
  const [isCreating, setIsCreating] = useState(false)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const navigate = useNavigate()

  useEffect(() => {
    loadGroups()

    // Auto-refresh every 10 seconds
    const interval = setInterval(loadGroups, 10000)
    return () => clearInterval(interval)
  }, [])

  const loadGroups = async () => {
    try {
      const fetchedGroups = await invoke<Group[]>('get_groups')
      setGroups(fetchedGroups)
    } catch (error) {
      console.error('Failed to load groups:', error)
      setErrorMessage('Failed to load groups: ' + String(error))
    } finally {
      setIsLoading(false)
    }
  }

  const handleCreateGroup = async () => {
    if (!groupName.trim()) return

    setIsCreating(true)
    setErrorMessage(null)

    try {
      const group = await invoke<Group>('create_group', {
        name: groupName.trim(),
        description: groupDescription.trim() || null
      })

      setGroups([...groups, group])
      setShowCreateDialog(false)
      setGroupName('')
      setGroupDescription('')

      // Navigate to the new group
      navigate(`/group/${group.id}`)
    } catch (error) {
      console.error('Failed to create group:', error)
      setErrorMessage('Failed to create group: ' + String(error))
    } finally {
      setIsCreating(false)
    }
  }


  return (
    <div className="flex flex-col h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white border-b border-gray-200 px-6 py-4">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-gray-900">Groups</h1>
            {localPeerId && (
              <p className="text-xs text-gray-500 font-mono truncate max-w-xs">
                {localPeerId}
              </p>
            )}
          </div>
          <div className="flex items-center space-x-4">
            <button
              onClick={() => navigate('/conversations')}
              className="btn-secondary text-sm"
            >
              ← Back
            </button>
            <button
              onClick={() => setShowCreateDialog(true)}
              className="btn-primary text-sm"
            >
              + New Group
            </button>
          </div>
        </div>
      </div>

      {/* Groups List */}
      <div className="flex-1 overflow-y-auto">
        {isLoading ? (
          <div className="flex items-center justify-center h-full">
            <div className="animate-spin rounded-full h-12 w-12 border-b-4 border-primary-500"></div>
          </div>
        ) : errorMessage ? (
          <div className="flex flex-col items-center justify-center h-full text-center px-6">
            <svg
              className="w-24 h-24 text-red-300 mb-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
              />
            </svg>
            <h2 className="text-xl font-semibold text-gray-900 mb-2">Error loading groups</h2>
            <p className="text-gray-600 mb-6">{errorMessage}</p>
            <button
              onClick={() => {
                setErrorMessage(null)
                setIsLoading(true)
                loadGroups()
              }}
              className="btn-primary"
            >
              Try Again
            </button>
          </div>
        ) : groups.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-center px-6">
            <svg
              className="w-24 h-24 text-gray-300 mb-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"
              />
            </svg>
            <h2 className="text-xl font-semibold text-gray-900 mb-2">No groups yet</h2>
            <p className="text-gray-600 mb-6">
              Create or join a group to get started
            </p>
            <button
              onClick={() => setShowCreateDialog(true)}
              className="btn-primary"
            >
              Create Your First Group
            </button>
          </div>
        ) : (
          <div className="divide-y divide-gray-200">
            {groups.map((group) => (
              <div
                key={group.id}
                onClick={() => navigate(`/group/${group.id}`)}
                className="px-6 py-4 hover:bg-gray-50 cursor-pointer transition-colors"
              >
                <div className="flex items-start justify-between">
                  <div className="flex items-start space-x-4 flex-1">
                    {/* Group Icon */}
                    <div className="w-12 h-12 rounded-full bg-blue-100 flex items-center justify-center flex-shrink-0">
                      <svg
                        className="w-6 h-6 text-blue-600"
                        fill="currentColor"
                        viewBox="0 0 20 20"
                      >
                        <path d="M13 6a3 3 0 11-6 0 3 3 0 016 0zM18 8a2 2 0 11-4 0 2 2 0 014 0zM14 15a4 4 0 00-8 0v3h8v-3zM6 8a2 2 0 11-4 0 2 2 0 014 0zM16 18v-3a5.972 5.972 0 00-.75-2.906A3.005 3.005 0 0119 15v3h-3zM4.75 12.094A5.973 5.973 0 004 15v3H1v-3a3 3 0 013.75-2.906z"></path>
                      </svg>
                    </div>

                    {/* Group Info */}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center space-x-2 mb-1">
                        <p className="text-base font-semibold text-gray-900 truncate">
                          {group.name}
                        </p>
                        {group.is_admin && (
                          <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800">
                            Admin
                          </span>
                        )}
                      </div>

                      {group.description && (
                        <p className="text-sm text-gray-600 truncate mb-1">
                          {group.description}
                        </p>
                      )}

                      <p className="text-xs text-gray-500">
                        {group.member_count} {group.member_count === 1 ? 'member' : 'members'}
                      </p>
                    </div>
                  </div>

                  {/* Created date */}
                  <p className="text-xs text-gray-500 ml-4">
                    {formatDate(group.created_at)}
                  </p>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Create Group Dialog */}
      {showCreateDialog && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-2xl shadow-2xl p-6 w-full max-w-md mx-4">
            <h2 className="text-xl font-bold text-gray-900 mb-4">Create Group</h2>

            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Group Name *
                </label>
                <input
                  type="text"
                  value={groupName}
                  onChange={(e) => setGroupName(e.target.value)}
                  placeholder="e.g., College Friends"
                  className="input-base"
                  autoFocus
                  disabled={isCreating}
                  onKeyPress={(e) => e.key === 'Enter' && handleCreateGroup()}
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Description (optional)
                </label>
                <textarea
                  value={groupDescription}
                  onChange={(e) => setGroupDescription(e.target.value)}
                  placeholder="e.g., Study group for CS classes"
                  className="input-base resize-none"
                  rows={3}
                  disabled={isCreating}
                />
              </div>

              {errorMessage && (
                <div className="bg-red-50 border border-red-200 rounded-lg p-3">
                  <p className="text-sm text-red-800">{errorMessage}</p>
                </div>
              )}
            </div>

            <div className="flex space-x-3 mt-6">
              <button
                onClick={() => {
                  setShowCreateDialog(false)
                  setGroupName('')
                  setGroupDescription('')
                  setErrorMessage(null)
                }}
                className="btn-secondary flex-1"
                disabled={isCreating}
              >
                Cancel
              </button>
              <button
                onClick={handleCreateGroup}
                disabled={!groupName.trim() || isCreating}
                className="btn-primary flex-1 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isCreating ? 'Creating...' : 'Create'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
