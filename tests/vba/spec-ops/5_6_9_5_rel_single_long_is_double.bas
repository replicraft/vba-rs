' INV: vbal.5.6.9.5, op.eq, op.lt
' SOURCE: spec ms-vbal 5.6.9.5 relational Single vs Long has effective type Double
' EXPECT: False
' EXPECT: True
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Dim s As Single, n As Long
    s = 16777216
    n = 16777217
    Debug.Print CStr(s = n)
    Debug.Print CStr(s < n)
    Debug.Print CStr(n = s)
    Debug.Print CStr(n > s)
End Sub
