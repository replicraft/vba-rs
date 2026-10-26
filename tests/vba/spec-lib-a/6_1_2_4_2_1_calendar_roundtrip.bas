' INV: vbal.6.1.2.4.2.1
' SOURCE: spec ms-vbal 6.1.2.4.2.1 Calendar property can be set to Gregorian and read back
' XFAIL: wrong pending
' EXPECT: calendar=0
' EXPECT: year=2020
Public Sub Main()
    Calendar = vbCalGreg
    Debug.Print "calendar=" & CStr(Calendar)
    Debug.Print "year=" & CStr(Year(DateSerial(2020, 3, 4)))
End Sub
