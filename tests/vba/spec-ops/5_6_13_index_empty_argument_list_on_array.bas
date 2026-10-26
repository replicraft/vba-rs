' INV: vbal.5.6.13, expr.index
' SOURCE: spec ms-vbal 5.6.13 an empty argument list on an array references the array itself
' EXPECT: 3
' EXPECT: 20
Private Function Total(ByRef arr() As Long) As Long
    Dim i As Long, acc As Long
    For i = LBound(arr) To UBound(arr)
        acc = acc + arr(i)
    Next i
    Total = acc
End Function
Public Sub Main()
    Dim a(1 To 3) As Long
    a(1) = 5: a(2) = 7: a(3) = 8
    Debug.Print CStr(UBound(a()))
    Debug.Print CStr(Total(a()))
End Sub
