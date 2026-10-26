' INV: vbal.6.1.2.11.1.4, func.chr
' SOURCE: spec ms-vbal 6.1.2.11.1.4 Chr outside 0..255 raises error 5 (negative value has no larger character set reading)
' ERROR: 5
Public Sub Main()
    Dim s As String
    s = Chr(-1)
    Debug.Print "unreachable"
End Sub
