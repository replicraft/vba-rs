' INV: vbal.6.1.2.7.1.3, func.isdate
' SOURCE: spec ms-vbal 6.1.2.7.1.3 IsDate true for Date values and Date-coercible strings
' EXPECT: date=True
' EXPECT: string=True
' EXPECT: badstring=False
' EXPECT: long=False
' EXPECT: null=False
Public Sub Main()
    Dim d As Date
    d = DateSerial(2020, 1, 2)
    Debug.Print "date=" & CStr(IsDate(d))
    Debug.Print "string=" & CStr(IsDate("2020-01-02"))
    Debug.Print "badstring=" & CStr(IsDate("not a date"))
    Debug.Print "long=" & CStr(IsDate(5&))
    Debug.Print "null=" & CStr(IsDate(Null))
End Sub
