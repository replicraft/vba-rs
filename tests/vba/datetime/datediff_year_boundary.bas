' INV: func.datediff
' SOURCE: spec ms-vbal 6.1.2.4.1.2 DateDiff year interval across December 31 to January 1
' EXPECT: 1
Public Sub Main()
    Debug.Print CStr(DateDiff("yyyy", DateSerial(1999, 12, 31), DateSerial(2000, 1, 1)))
End Sub
