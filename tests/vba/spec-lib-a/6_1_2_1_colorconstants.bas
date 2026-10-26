' INV: vbal.6.1.2.1
' SOURCE: spec ms-vbal 6.1.2.1 ColorConstants values
' XFAIL: wrong pending
' EXPECT: vbBlack=0
' EXPECT: vbBlue=16711680
' EXPECT: vbCyan=16776960
' EXPECT: vbGreen=65280
' EXPECT: vbMagenta=16711935
' EXPECT: vbRed=255
' EXPECT: vbWhite=16777215
' EXPECT: vbYellow=65535
Public Sub Main()
    Debug.Print "vbBlack=" & CStr(vbBlack)
    Debug.Print "vbBlue=" & CStr(vbBlue)
    Debug.Print "vbCyan=" & CStr(vbCyan)
    Debug.Print "vbGreen=" & CStr(vbGreen)
    Debug.Print "vbMagenta=" & CStr(vbMagenta)
    Debug.Print "vbRed=" & CStr(vbRed)
    Debug.Print "vbWhite=" & CStr(vbWhite)
    Debug.Print "vbYellow=" & CStr(vbYellow)
End Sub
