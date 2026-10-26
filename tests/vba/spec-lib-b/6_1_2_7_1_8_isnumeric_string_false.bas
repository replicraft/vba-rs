' INV: vbal.6.1.2.7.1.8, func.isnumeric
' SOURCE: spec ms-vbal 6.1.2.7.1.8 IsNumeric otherwise False (String is not a listed value type)
' XFAIL: wrong pending
' EXPECT: string=False
Public Sub Main()
    Debug.Print "string=" & CStr(IsNumeric("123"))
End Sub
