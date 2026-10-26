' INV: func.cbyte
' SOURCE: derived outside 0 to 255 is an error (page); Overflow is error 6
' ERROR: 6
Public Sub Main()
    Debug.Print CByte(256)
End Sub
