' INV: vbal.6.1.2.3.1.14, func.cverr
' SOURCE: spec ms-vbal 6.1.2.3.1.14 CVErr returns an Error data value
' EXPECT: iserr5=True
' EXPECT: iserr0=True
' EXPECT: iserrMax=True
' EXPECT: str5=Error 5
' EXPECT: str0=Error 0
' EXPECT: strMax=Error 65535
' EXPECT: frac=Error 2
' EXPECT: strArg=Error 11
' EXPECT: vartype=10
Public Sub Main()
    Debug.Print "iserr5=" & CStr(IsError(CVErr(5)))
    Debug.Print "iserr0=" & CStr(IsError(CVErr(0)))
    Debug.Print "iserrMax=" & CStr(IsError(CVErr(65535)))
    Debug.Print "str5=" & CStr(CVErr(5))
    Debug.Print "str0=" & CStr(CVErr(0))
    Debug.Print "strMax=" & CStr(CVErr(65535))
    Debug.Print "frac=" & CStr(CVErr(2.4))
    Debug.Print "strArg=" & CStr(CVErr("11"))
    Debug.Print "vartype=" & CStr(VarType(CVErr(3)))
End Sub
