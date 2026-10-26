' INV: vbal.6.1.2.11.1.11, func.formatdatetime
' SOURCE: spec ms-vbal 6.1.2.11.1.11 VbDateTimeFormat constant values, return type String
' EXPECT: gd=0
' EXPECT: ld=1
' EXPECT: sd=2
' EXPECT: lt=3
' EXPECT: st=4
' EXPECT: t=String
Public Sub Main()
    Debug.Print "gd=" & CStr(vbGeneralDate)
    Debug.Print "ld=" & CStr(vbLongDate)
    Debug.Print "sd=" & CStr(vbShortDate)
    Debug.Print "lt=" & CStr(vbLongTime)
    Debug.Print "st=" & CStr(vbShortTime)
    Debug.Print "t=" & TypeName(FormatDateTime(DateSerial(2020, 1, 2), vbShortDate))
End Sub
