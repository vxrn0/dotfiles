// Lean-context adapter for OpenCode V2, based on the user-supplied plugin.
// It applies only to DeepSeek V4.1 Flash and changes the outgoing request window.

const KEEP_TURNS = 2
const MODELS = /deepseek-v4\.1-flash/i
const CLEARED = '[Old tool result content cleared]'

function isSignedReasoning(part) {
  return Boolean(
    part.encrypted ||
    part.metadata?.anthropic?.signature ||
    part.providerMetadata?.anthropic?.signature
  )
}

function withoutMessageID(message, content) {
  const { id: _id, ...rest } = message
  return { ...rest, content }
}

function transform(event) {
  if (!MODELS.test(event.model?.id ?? '')) return

  const messages = event.messages
  const userIndexes = []
  for (let i = 0; i < messages.length; i++) {
    if (messages[i].role === 'user') userIndexes.push(i)
  }
  const lastUser = userIndexes.at(-1) ?? -1
  if (lastUser < 0) return

  const keepFrom = userIndexes.length >= KEEP_TURNS
    ? userIndexes[userIndexes.length - KEEP_TURNS]
    : 0

  for (let i = 0; i < lastUser; i++) {
    const message = messages[i]
    if (!Array.isArray(message.content)) continue

    if (message.role === 'tool' && i < keepFrom) {
      let changed = false
      const content = message.content.map((part) => {
        if (part.type !== 'tool-result' || part.result?.type === 'error') return part
        changed = true
        return { ...part, result: { type: 'text', value: CLEARED } }
      })
      if (changed) messages[i] = withoutMessageID(message, content)
      continue
    }

    if (message.role === 'assistant') {
      let changed = false
      const content = message.content.map((part) => {
        if (part.type !== 'reasoning' || !part.text || isSignedReasoning(part)) return part
        changed = true
        return { ...part, text: '' }
      })
      if (changed) messages[i] = withoutMessageID(message, content)
    }
  }
}

export default {
  id: 'lean-context',
  async setup(ctx) {
    await ctx.session.hook('context', transform)
    await ctx.session.hook('compaction', transform)
  },
}
