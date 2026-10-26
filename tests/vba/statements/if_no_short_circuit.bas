' INV: stmt.if-then-else, op.and
' SOURCE: spec ms-vbal 5.6.9.2 simple data operators evaluate both operands, 5.6.9.8.2 And operator
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
