' INV: lit.string
' SOURCE: spec ms-vbal 3.3.4 string tokens doubled quote
' EXPECT: say "hi"
' EXPECT:  1 
' EXPECT: ""
Public Sub Main()
    Debug.Print "say ""hi"""
    Debug.Print Len("""")
    Debug.Print """"""
End Sub
