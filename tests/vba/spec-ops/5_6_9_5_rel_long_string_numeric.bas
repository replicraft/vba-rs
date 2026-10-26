' INV: vbal.5.6.9.5, op.lt, op.eq
' SOURCE: spec ms-vbal 5.6.9.5 relational Long vs String compares numerically
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim n As Long
    n = 70000
    Debug.Print CStr(n = "70000")
    Debug.Print CStr(n > "5")
    Debug.Print CStr("9" < n)
    Debug.Print CStr(CLng(2) < "10")
End Sub
