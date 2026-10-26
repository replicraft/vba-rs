' INV: vbal.6.1.2.7.1.8, func.isnumeric
' SOURCE: spec ms-vbal 6.1.2.7.1.8 IsNumeric otherwise False (Empty is not a listed value type)
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: IsNumeric accepts numeric strings and Empty
' EXPECT: empty=False
Public Sub Main()
    Debug.Print "empty=" & CStr(IsNumeric(Empty))
End Sub
