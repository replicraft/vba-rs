' INV: func.msgbox
' SOURCE: unverified design of the headless host: MsgBox is error 70 (not available); VBA on a desktop would show a dialog
' ERROR: 70
Public Sub Main()
    Debug.Print MsgBox("hi")
End Sub
