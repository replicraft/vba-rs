' INV: func.trim
' SOURCE: derived Trim removes spaces only, not tabs
' EXPECT: 3
' EXPECT: 
Public Sub Main()
    Debug.Print CStr(Len(Trim(vbTab & "a" & vbTab)))
    Debug.Print CStr(Trim("   "))
End Sub
