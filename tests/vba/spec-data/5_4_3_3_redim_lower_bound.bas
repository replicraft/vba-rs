' INV: vbal.5.4.3.3, stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 dynamic bounds with lower bound
' EXPECT: 2,5
' EXPECT: 1,2,2,2
Public Sub Main()
    Dim a() As Integer
    Dim b() As Integer
    ReDim a(2 To 5)
    Debug.Print CStr(LBound(a)) & "," & CStr(UBound(a))
    ReDim b(1 To 2, 3 To 4)
    Debug.Print CStr(LBound(b, 1)) & "," & CStr(UBound(b, 1)) & "," & CStr(LBound(b, 2) - 1) & "," & CStr(UBound(b, 2) - 2)
End Sub
