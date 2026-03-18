
'use strict'

if (process.env.NODE_ENV === 'production') {
  module.exports = require('./prisma-ast.cjs.production.min.js')
} else {
  module.exports = require('./prisma-ast.cjs.development.js')
}
