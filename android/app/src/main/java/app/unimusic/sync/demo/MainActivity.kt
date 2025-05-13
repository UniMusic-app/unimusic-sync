package app.unimusic.sync.demo

import app.unimusic.sync.demo.ui.theme.UniMusicSyncTheme
import app.unimusic.sync.UniMusicSync

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
  var author = "Unknown"

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)

    CoroutineScope(Dispatchers.IO).launch {
      println("CREATING IROH MANAGER...")
      val irohManager = UniMusicSync.create(applicationContext.filesDir.path)
      println("CREATED IROH MANAGER")
      author = irohManager.irohManager.getAuthor()
      print("GOT AUTHOR: ")

      render()
    }

    render()
  }

  private fun render() {
    setContent {
      UniMusicSyncTheme {
        // A surface container using the 'background' color from the theme
        Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
          Greeting(author)
        }
      }
    }
  }
}

@Composable
fun Greeting(name: String, modifier: Modifier = Modifier) {
  Text(text = "Hello $name!", modifier = modifier)
}

@Preview(showBackground = true)
@Composable
fun GreetingPreview() {
  UniMusicSyncTheme { Greeting("Android") }
}
