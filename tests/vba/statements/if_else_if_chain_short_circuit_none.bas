' INV: stmt.if-then-else, op.or
' SOURCE: spec ms-vbal 5.6.9.2 simple data operators evaluate both operands, 5.6.9.8.3 Or operator
' EXPECT:  12 
Dim g As Long
Function T() As Boolean
    g = g + 1
    T = True
End Function
Public Sub Main()
    If T() Or T() Then g = g + 10
    Debug.Print g
End Sub
