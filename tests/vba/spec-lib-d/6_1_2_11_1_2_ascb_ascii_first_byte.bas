' INV: vbal.6.1.2.11.1.2
' SOURCE: spec ms-vbal 6.1.2.11.1.2 AscB first byte; for a 7-bit character exactly one of the two bytes is its code
' EXPECT: ok=True
' EXPECT: t=Integer
Public Sub Main()
    Dim r As Integer
    r = AscB("A")
    Debug.Print "ok=" & CStr(r = 65 Or r = 0)
    Debug.Print "t=" & TypeName(AscB("A"))
End Sub
