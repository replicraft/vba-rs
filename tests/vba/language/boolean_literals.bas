' INV: lit.boolean
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: TrueFalse
' EXPECT: -1 
Public Sub Main()
    Debug.Print True; False
    Debug.Print CInt(True)
End Sub
