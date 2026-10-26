' INV: stmt.if-then-else, op.or
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
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
