' INV: op.mul, type.double
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' ERROR: 6
Public Sub Main()
    Debug.Print 1E308 * 10
End Sub
