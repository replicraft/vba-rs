' INV: func.cbool
' SOURCE: spec ms-vbal 5.5.1.2.4 String to Boolean via Double coercion
' ERROR: 13
Public Sub Main()
    Debug.Print CBool("abc")
End Sub
