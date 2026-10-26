' INV: vbal.6.1.1.11
' SOURCE: spec ms-vbal 6.1.1.11 VbMsgBoxResult values
' EXPECT: vbAbort=3
' EXPECT: vbCancel=2
' EXPECT: vbIgnore=5
' EXPECT: vbNo=7
' EXPECT: vbOK=1
' EXPECT: vbRetry=4
' EXPECT: vbYes=6
Public Sub Main()
    Debug.Print "vbAbort=" & CStr(vbAbort)
    Debug.Print "vbCancel=" & CStr(vbCancel)
    Debug.Print "vbIgnore=" & CStr(vbIgnore)
    Debug.Print "vbNo=" & CStr(vbNo)
    Debug.Print "vbOK=" & CStr(vbOK)
    Debug.Print "vbRetry=" & CStr(vbRetry)
    Debug.Print "vbYes=" & CStr(vbYes)
End Sub
