' INV: vbal.5.6.2.1, expr.index
' SOURCE: spec ms-vbal 5.6.2.1 Evaluation to a data value, an argument outside the array dimension raises error 9
' EXPECT: in range=3
' ERROR: 9
Public Sub Main()
    Dim a(1 To 3) As Long
    Dim x As Long
    a(3) = 3
    Debug.Print "in range=" & CStr(a(3))
    x = a(4)
    Debug.Print "not reached"
End Sub
