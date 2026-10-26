' INV: vbal.5.6.13, expr.index
' SOURCE: spec ms-vbal 5.6.13 index on array with positional arguments equal to the rank references an element
' EXPECT: 23
' EXPECT: 12
' EXPECT: 99
Public Sub Main()
    Dim m(1 To 2, 1 To 3) As Long
    m(2, 3) = 23
    m(1, 2) = 12
    Debug.Print CStr(m(2, 3))
    Debug.Print CStr(m(1, 2))
    m(1, 1) = 99
    Debug.Print CStr(m(1, 1))
End Sub
