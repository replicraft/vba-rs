' INV: lit.string
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: say "hi"
' EXPECT:  1 
' EXPECT: ""
Public Sub Main()
    Debug.Print "say ""hi"""
    Debug.Print Len("""")
    Debug.Print """"""
End Sub
