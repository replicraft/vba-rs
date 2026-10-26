' INV: vbal.6.1.2.3.1.20, func.str
' SOURCE: spec ms-vbal 6.1.2.3.1.20 Str of an Error value is "Error n"
' XFAIL: wrong pending
' EXPECT: err=[Error 5]
Public Sub Main()
    Debug.Print "err=[" & Str(CVErr(5)) & "]"
End Sub
