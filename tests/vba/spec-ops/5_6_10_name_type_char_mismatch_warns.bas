' INV: vbal.5.6.10
' SOURCE: spec ms-vbal 5.6.10 type character not matching the declared type makes the name invalid (spec-strict: a warning by default, the program runs)
' EXPECT: 1
' EXPECT: ok
Public Sub Main()
    Dim n As Long
    n = 1
    Debug.Print CStr(n$)
    Debug.Print "ok"
End Sub
