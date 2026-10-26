' INV: vbal.5.4.3.8, stmt.let
' SOURCE: spec ms-vbal 5.4.3.8 Let keyword optional
' EXPECT: 3 4
Public Sub Main()
    Dim a As Long, b As Long
    Let a = 3
    b = 4
    Debug.Print CStr(a) & " " & CStr(b)
End Sub
