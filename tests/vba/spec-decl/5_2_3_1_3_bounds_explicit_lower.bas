' INV: vbal.5.2.3.1.3
' SOURCE: spec ms-vbal 5.2.3.1.3 lower-bound To upper-bound, multiple dimensions
' EXPECT: 1 5 -3 3 0 4
Dim a(1 To 5, -3 To 3, 4) As Long
Public Sub Main()
    Debug.Print CStr(LBound(a, 1)) & " " & CStr(UBound(a, 1)) & " " & CStr(LBound(a, 2)) & " " & CStr(UBound(a, 2)) & " " & CStr(LBound(a, 3)) & " " & CStr(UBound(a, 3))
End Sub
