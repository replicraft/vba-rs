' INV: vbal.6.1.1.1
' SOURCE: spec ms-vbal 6.1.1.1 FormShowConstants values
' XFAIL: wrong pending
' EXPECT: vbModal=1
' EXPECT: vbModeless=0
Public Sub Main()
    Debug.Print "vbModal=" & CStr(vbModal)
    Debug.Print "vbModeless=" & CStr(vbModeless)
End Sub
