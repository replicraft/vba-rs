' INV: func.cbool
' SOURCE: derived nonzero is True (type-conversion-functions page)
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Debug.Print CStr(CBool(-1))
    Debug.Print CStr(CBool(2.5))
    Debug.Print CStr(CBool("True"))
    Debug.Print CStr(CBool("False"))
End Sub
