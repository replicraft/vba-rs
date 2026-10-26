' INV: vbal.6.1.2.3.1.2, vbal.5.5.1.2.2, func.cbyte
' SOURCE: spec ms-vbal 6.1.2.3.1.2 CByte of True is 255 (Let-coercion Boolean to Byte)
' XFAIL: wrong pending
' EXPECT: true=255
' EXPECT: false=0
Public Sub Main()
    Debug.Print "true=" & CStr(CByte(True))
    Debug.Print "false=" & CStr(CByte(False))
End Sub
