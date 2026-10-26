' INV: stmt.function
' SOURCE: derived unbounded recursion ends with a classified limit, not a crash
' ERROR: limit
Sub R()
    R
End Sub
Public Sub Main()
    R
End Sub
