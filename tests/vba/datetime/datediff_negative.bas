' INV: func.datediff
' SOURCE: spec ms-vbal 6.1.2.4.1.2 DateDiff negative when Date1 is later
' EXPECT: -60
Public Sub Main()
    Debug.Print CStr(DateDiff("d", DateSerial(2000, 3, 1), DateSerial(2000, 1, 1)))
End Sub
