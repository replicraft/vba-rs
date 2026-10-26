' INV: func.chr
' SOURCE: derived Chr outside 0..255 is error 5
' ERROR: 5
Public Sub Main()
    Debug.Print Chr(256)
End Sub
