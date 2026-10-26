' INV: func.trim
' SOURCE: spec ms-vbal 6.1.2.11.1.23 Trim removes spaces (vbTab is Chr$(9))
' EXPECT: 3
' EXPECT: 
Public Sub Main()
    Debug.Print CStr(Len(Trim(vbTab & "a" & vbTab)))
    Debug.Print CStr(Trim("   "))
End Sub
