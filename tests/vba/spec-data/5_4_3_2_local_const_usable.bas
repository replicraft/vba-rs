' INV: vbal.5.4.3.2, stmt.const
' SOURCE: spec ms-vbal 5.4.3.2 local constant declaration
' EXPECT: 7
Public Sub Main()
    Const a = 3, b = 4
    Debug.Print CStr(a + b)
End Sub
