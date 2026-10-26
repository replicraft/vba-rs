' INV: stmt.if-then-else, op.and
' SOURCE: derived VBA evaluates both operands of And (no short circuit)
' EXPECT:  1 
Dim g As Long
Function F() As Boolean
    g = g + 1
    F = True
End Function
Public Sub Main()
    If False And F() Then Debug.Print "x"
    Debug.Print g
End Sub
