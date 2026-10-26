' INV: obj.debug
' SOURCE: derived numbers print with a leading sign column and a trailing space (Debug.Print / Print # layout)
' EXPECT:  5 
' EXPECT: -5 
' EXPECT:  1.5 
' EXPECT: True
' EXPECT: s
Public Sub Main()
    Debug.Print 5
    Debug.Print -5
    Debug.Print 1.5
    Debug.Print True
    Debug.Print "s"
End Sub
