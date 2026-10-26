' INV: vbal.6.1.1.12
' SOURCE: spec ms-vbal 6.1.1.12 VbMsgBoxStyle values
' EXPECT: vbAbortRetryIgnore=2
' EXPECT: vbApplicationModal=0
' EXPECT: vbCritical=16
' EXPECT: vbDefaultButton1=0
' EXPECT: vbDefaultButton2=256
' EXPECT: vbDefaultButton3=512
' EXPECT: vbDefaultButton4=768
' EXPECT: vbExclamation=48
' EXPECT: vbInformation=64
' EXPECT: vbMsgBoxHelpButton=16384
' EXPECT: vbMsgBoxRight=524288
' EXPECT: vbMsgBoxRtlReading=1048576
' EXPECT: vbMsgBoxSetForeground=65536
' EXPECT: vbOKCancel=1
' EXPECT: vbOKOnly=0
' EXPECT: vbQuestion=32
' EXPECT: vbRetryCancel=5
' EXPECT: vbSystemModal=4096
' EXPECT: vbYesNo=4
' EXPECT: vbYesNoCancel=3
Public Sub Main()
    Debug.Print "vbAbortRetryIgnore=" & CStr(vbAbortRetryIgnore)
    Debug.Print "vbApplicationModal=" & CStr(vbApplicationModal)
    Debug.Print "vbCritical=" & CStr(vbCritical)
    Debug.Print "vbDefaultButton1=" & CStr(vbDefaultButton1)
    Debug.Print "vbDefaultButton2=" & CStr(vbDefaultButton2)
    Debug.Print "vbDefaultButton3=" & CStr(vbDefaultButton3)
    Debug.Print "vbDefaultButton4=" & CStr(vbDefaultButton4)
    Debug.Print "vbExclamation=" & CStr(vbExclamation)
    Debug.Print "vbInformation=" & CStr(vbInformation)
    Debug.Print "vbMsgBoxHelpButton=" & CStr(vbMsgBoxHelpButton)
    Debug.Print "vbMsgBoxRight=" & CStr(vbMsgBoxRight)
    Debug.Print "vbMsgBoxRtlReading=" & CStr(vbMsgBoxRtlReading)
    Debug.Print "vbMsgBoxSetForeground=" & CStr(vbMsgBoxSetForeground)
    Debug.Print "vbOKCancel=" & CStr(vbOKCancel)
    Debug.Print "vbOKOnly=" & CStr(vbOKOnly)
    Debug.Print "vbQuestion=" & CStr(vbQuestion)
    Debug.Print "vbRetryCancel=" & CStr(vbRetryCancel)
    Debug.Print "vbSystemModal=" & CStr(vbSystemModal)
    Debug.Print "vbYesNo=" & CStr(vbYesNo)
    Debug.Print "vbYesNoCancel=" & CStr(vbYesNoCancel)
End Sub
