' INV: vbal.6.1.2.8.1.13, func.msgbox
' SOURCE: spec ms-vbal 6.1.2.8.1.13 MsgBox modality group and further option constants
' EXPECT: 0 4096
' EXPECT: 16384 65536 524288 1048576
Public Sub Main()
    Debug.Print vbApplicationModal & " " & vbSystemModal
    Debug.Print vbMsgBoxHelpButton & " " & vbMsgBoxSetForeground & " " & vbMsgBoxRight & " " & vbMsgBoxRtlReading
End Sub
