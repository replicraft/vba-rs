' INV: vbal.5.2.3.1.3
' SOURCE: spec ms-vbal 5.2.3.1.3 lower bound equal to upper bound is valid
' EXPECT: 4 4
Dim a(4 To 4) As Long
Public Sub Main()
    Debug.Print CStr(LBound(a)) & " " & CStr(UBound(a))
End Sub
