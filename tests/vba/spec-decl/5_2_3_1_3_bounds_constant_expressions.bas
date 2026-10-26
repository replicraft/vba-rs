' INV: vbal.5.2.3.1.3
' SOURCE: spec ms-vbal 5.2.3.1.3 bounds are constant-expressions
' EXPECT: 2 8
Private Const N As Long = 5
Dim a(N - 3 To N * 2 - 2) As Long
Public Sub Main()
    Debug.Print CStr(LBound(a)) & " " & CStr(UBound(a))
End Sub
