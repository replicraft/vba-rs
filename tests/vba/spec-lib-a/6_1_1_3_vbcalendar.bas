' INV: vbal.6.1.1.3
' SOURCE: spec ms-vbal 6.1.1.3 VbCalendar values
' EXPECT: vbCalGreg=0
' EXPECT: vbCalHijri=1
Public Sub Main()
    Debug.Print "vbCalGreg=" & CStr(vbCalGreg)
    Debug.Print "vbCalHijri=" & CStr(vbCalHijri)
End Sub
