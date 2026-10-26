' INV: vbal.6.1.2.7.1.8, func.isnumeric
' SOURCE: spec ms-vbal 6.1.2.7.1.8 IsNumeric otherwise False (Empty is not a listed value type)
' XFAIL: wrong pending
' EXPECT: empty=False
Public Sub Main()
    Debug.Print "empty=" & CStr(IsNumeric(Empty))
End Sub
