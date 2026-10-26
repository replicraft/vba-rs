' INV: vbal.6.1.2.7.1.8, func.isnumeric
' SOURCE: spec ms-vbal 6.1.2.7.1.8 IsNumeric otherwise False (Null, Date, Error not listed)
' EXPECT: null=False
' EXPECT: date=False
' EXPECT: error=False
Public Sub Main()
    Debug.Print "null=" & CStr(IsNumeric(Null))
    Debug.Print "date=" & CStr(IsNumeric(DateSerial(2020, 1, 2)))
    Debug.Print "error=" & CStr(IsNumeric(CVErr(5)))
End Sub
